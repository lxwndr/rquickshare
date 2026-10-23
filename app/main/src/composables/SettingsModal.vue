<script setup lang="ts">
import { utils } from '../vue_lib';
import { PropType } from 'vue';
import { TauriVM } from '../vue_lib/helper/ParamsHelper';
import { useI18n } from "vue-i18n";
import { setThemePreference, themePreference } from "../theme";
import type { ThemePreference } from "../theme";

const i18n = useI18n({ useScope: "global" });
const { t } = i18n;

const props = defineProps({
	vm: {
		type: Object as PropType<TauriVM>,
		required: true
	}
});

const emit = defineEmits(['close']);

function changeLanguage(event: Event) {
	const locale = (event.target as HTMLSelectElement).value;
	i18n.locale.value = locale;
	localStorage.setItem("rquickshare-locale", locale);
}

function changeTheme(event: Event) {
	setThemePreference((event.target as HTMLSelectElement).value as ThemePreference);
}

function openDownloadPicker() {
	props.vm.dialogOpen({
		title: t('labels.selectFileDestination'),
		directory: true,
		multiple: false,
	}).then(async (el) => {
		if (el === null) {
			return;
		}

		await utils.setDownloadPath(props.vm, el as string);
	});
}
</script>

<template>
	<div v-if="vm.settingsOpen"
		class="absolute z-10 w-full h-full flex justify-center items-center bg-black bg-opacity-25">
		<div class="bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 rounded-xl shadow-xl p-4 w-[24rem]">
			<div class="flex flex-row justify-between items-center">
				<h3 class="font-medium text-xl">
					{{ $t('titles.settings') }}
				</h3>
				<div class="btn px-3 rounded-xl active:scale-95 transition duration-150 ease-in-out"
					@click="emit('close')">
					{{ $t('buttons.close') }}
				</div>
			</div>
			<div class="py-4 flex flex-col">
				<div class="form-control rounded-xl p-3">
					<label for="language-preference" class="label-text text-gray-900 dark:text-gray-100 mb-2">{{ $t('titles.language') }}</label>
					<select id="language-preference" :value="i18n.locale.value" @change="changeLanguage"
						class="w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 p-2 focus:outline-none">
						<option value="en">Inglês</option>
						<option value="pt-BR">Português (Brasil)</option>
						<option value="fr">Francês</option>
						<option value="de">Alemão</option>
						<option value="it">Italiano</option>
						<option value="es">Espanhol</option>
					</select>
				</div>
				<div class="form-control rounded-xl p-3">
					<label for="theme-preference" class="label-text text-gray-900 dark:text-gray-100 mb-2">{{ $t('labels.theme') }}</label>
					<select id="theme-preference" :value="themePreference" @change="changeTheme"
						class="w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 p-2 focus:outline-none">
						<option value="system">{{ $t('labels.themeSystem') }}</option>
						<option value="light">{{ $t('labels.themeLight') }}</option>
						<option value="dark">{{ $t('labels.themeDark') }}</option>
					</select>
				</div>
				<div class="form-control hover:bg-gray-500 hover:bg-opacity-10 rounded-xl p-3">
					<label class="cursor-pointer flex flex-row justify-between items-center"
						@click="utils.setAutoStart(vm, !vm.autostart)">
						<span class="label-text text-gray-900 dark:text-gray-100">{{ $t('labels.startOnBoot') }}</span>
						<input type="checkbox" :checked="vm.autostart" class="checkbox focus:outline-none">
					</label>
				</div>
				<div class="form-control hover:bg-gray-500 hover:bg-opacity-10 rounded-xl p-3">
					<label class="cursor-pointer flex flex-row justify-between items-center"
						@click="utils.setRealClose(vm, !vm.realclose)">
						<span class="label-text">{{ $t('labels.runningOnClose') }}</span>
						<input type="checkbox" :checked="!vm.realclose" class="checkbox focus:outline-none">
					</label>
				</div>
				<div class="form-control hover:bg-gray-500 hover:bg-opacity-10 rounded-xl p-3">
					<label class="cursor-pointer flex flex-row justify-between items-center"
						@click="utils.setStartMinimized(vm, !vm.startminimized)">
						<span class="label-text">{{ $t('labels.startMinimized') }}</span>
						<input type="checkbox" :checked="vm.startminimized" class="checkbox focus:outline-none">
					</label>
				</div>
				<div class="form-control hover:bg-gray-500 hover:bg-opacity-10 rounded-xl p-3">
					<label class="cursor-pointer flex flex-col items-start" @click="openDownloadPicker()">
						<span class="">{{ $t('labels.selectDownloadFolder') }}</span>
						<span class="overflow-hidden whitespace-nowrap text-ellipsis text-xs max-w-80 text-gray-600 dark:text-gray-300">
							> {{ vm.downloadPath ?? $t('labels.usersFolder') }}
						</span>
					</label>
				</div>
			</div>
		</div>
	</div>
</template>